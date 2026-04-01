import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { DbManagementComponent } from './db-management.component';
import { DatabasesComponent } from './databases/databases.component';
import { TablesComponent } from './tables/tables.component';
import { PermissionGuard } from '../../../@core/guards/permission.guard';

const routes: Routes = [
  {
    path: '',
    component: DbManagementComponent,
    canActivate: [PermissionGuard],
    data: {
      permission: 'menu:db-management'
    },
    children: [
      {
        path: 'databases',
        component: DatabasesComponent,
        canActivate: [PermissionGuard],
        data: {
          permission: 'menu:db-management:databases'
        }
      },
      {
        path: 'tables',
        component: TablesComponent,
        canActivate: [PermissionGuard],
        data: {
          permission: 'menu:db-management:tables'
        }
      },
      {
        path: '',
        redirectTo: 'databases',
        pathMatch: 'full'
      }
    ]
  }
];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule]
})
export class DbManagementRoutingModule { }
