import { Component, OnInit } from '@angular/core';
import { Router, ActivatedRoute } from '@angular/router';
import { NbToastrService } from '@nebular/theme';
import { AuthService } from '../../@core/data/auth.service';

@Component({
  selector: 'ngx-login',
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.scss']
})
export class LoginComponent implements OnInit {
  submitted = false;
  user = {
    username: '',
    password: ''
  };
  rememberMe = false;
  errors: string[] = [];
  messages: string[] = [];
  showMessages = false;
  returnUrl: string;

  constructor(
    protected router: Router,
    private route: ActivatedRoute,
    private authService: AuthService,
    private toastrService: NbToastrService
  ) {}

  ngOnInit() {
    const rawReturnUrl = this.route.snapshot.queryParams['returnUrl'];
    this.returnUrl = this.authService.normalizeReturnUrl(rawReturnUrl);
    
    // Load saved username if remember me was checked
    const savedUsername = localStorage.getItem('remembered_username');
    if (savedUsername) {
      this.user.username = savedUsername;
      this.rememberMe = true;
    }
    
    // If already logged in, redirect to return URL using absolute navigation
    if (this.authService.isAuthenticated()) {
      this.router.navigateByUrl(this.returnUrl, { replaceUrl: true });
    }
  }

  login(): void {
    this.errors = [];
    this.messages = [];
    this.submitted = true;

    if (!this.user.username || !this.user.password) {
      this.errors.push('Username and password are required!');
      this.submitted = false;
      return;
    }

    this.authService.login(this.user).subscribe({
      next: (response) => {
        this.submitted = false;
        
        // Handle remember me functionality
        if (this.rememberMe) {
          localStorage.setItem('remembered_username', this.user.username);
        } else {
          localStorage.removeItem('remembered_username');
        }
        
        // Check if first login - redirect to password change page
        if (response.user.first_log) {
          // First login - redirect to user settings page without showing toast
          // The toast will be shown in the user-settings page
          setTimeout(() => {
            this.router.navigate(['/pages/user-settings'], { replaceUrl: true });
          }, 500);
        } else {
          // Normal login - show welcome message and navigate to return URL
          this.toastrService.success('Welcome back!', 'Login Successful');
          setTimeout(() => {
            this.router.navigateByUrl(this.returnUrl, { replaceUrl: true });
          }, 500);
        }
      },
      error: (error) => {
        this.submitted = false;
        // Show error in alert (form validation errors use alert, API errors use alert too for consistency)
        const errorMessage = error.error?.message || 'Login failed. Please check your credentials.';
        this.errors = [errorMessage];
        this.showMessages = true;
        // Don't show toast for API errors since we already show alert
        // this.toastrService.danger(errorMessage, 'Login Failed');
      }
    });
  }
}
