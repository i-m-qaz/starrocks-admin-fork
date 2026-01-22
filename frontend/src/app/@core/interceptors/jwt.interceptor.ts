import { Injectable } from '@angular/core';
import {
  HttpRequest,
  HttpHandler,
  HttpEvent,
  HttpInterceptor,
  HttpErrorResponse,
} from '@angular/common/http';
import { Router } from '@angular/router';
import { Observable, throwError } from 'rxjs';
import { catchError } from 'rxjs/operators';
import { AuthService } from '../data/auth.service';
import { NbToastrService } from '@nebular/theme';

@Injectable()
export class JwtInterceptor implements HttpInterceptor {
  constructor(
    private authService: AuthService,
    private toastrService: NbToastrService,
    private router: Router,
  ) {}

  intercept(request: HttpRequest<unknown>, next: HttpHandler): Observable<HttpEvent<unknown>> {
    // Add authorization header with JWT token if available
    const token = this.authService.token;
    if (token) {
      request = request.clone({
        setHeaders: {
          Authorization: `Bearer ${token}`,
        },
      });
    }

    return next.handle(request).pipe(
      catchError((error: HttpErrorResponse) => {
        if (error.status === 401) {
          const errorMessage = error.error?.message || 'Unauthorized';
          const isAuthError = errorMessage.includes('Missing authorization header') ||
                              errorMessage.includes('Invalid authorization header') ||
                              errorMessage.includes('JWT verification failed') ||
                              errorMessage.includes('Token expired') ||
                              errorMessage.includes('Invalid credentials');

          const isFirstLoginError = errorMessage.includes('首次登录必须修改密码');

          if (this.authService.isAuthenticated()) {
            if (isAuthError) {
              this.toastrService.danger('登录已过期，请重新登录', '认证失败');
              const safeUrl = this.authService.normalizeReturnUrl(this.router.url);
              this.authService.logout({ returnUrl: safeUrl });
            } else if (!isFirstLoginError) {
              this.toastrService.danger(errorMessage, '无权限');
            }
          }
        }
        return throwError(() => error);
      }),
    );
  }
}

